# Copyright Materialize, Inc. and contributors. All rights reserved.
#
# Use of this software is governed by the Business Source License
# included in the LICENSE file at the root of this repository.
#
# As of the Change Date specified in that file, in accordance with
# the Business Source License, use of this software will be governed
# by the Apache License, Version 2.0.

$ set-arg-default single-replica-cluster=quickstart

# Test that debezium deduplication works correctly in the presence of multiple partitions

$ set key-schema={"type": "record", "name": "row", "fields": [{"name": "a", "type": "long"}]}

$ set schema={
    "type": "record",
    "name": "envelope",
    "fields": [
      {
        "name": "before",
        "type": [
          {
            "name": "row",
            "type": "record",
            "fields": [
              {"name": "a", "type": "long"},
              {"name": "b", "type": "long"}
            ]
          },
          "null"
        ]
      },
      { "name": "after", "type": ["row", "null"] },
      { "name": "op", "type": "string" },
      {
        "name": "source",
        "type": {
          "type": "record",
          "name": "Source",
          "namespace": "io.debezium.connector.mysql",
          "fields": [
            {
              "name": "file",
              "type": "string"
            },
            {
              "name": "pos",
              "type": "long"
            },
            {
              "name": "row",
              "type": "int"
            },
            {
              "name": "snapshot",
              "type": [
                {
                  "type": "boolean",
                  "connect.default": false
                },
                "null"
              ],
              "default": false
            }
          ],
          "connect.name": "io.debezium.connector.mysql.Source"
        }
      }
    ]
  }

$ kafka-create-topic topic=data partitions=3


> CREATE CONNECTION IF NOT EXISTS csr_conn TO CONFLUENT SCHEMA REGISTRY (
    URL '${testdrive.schema-registry-url}'
  );

> CREATE CONNECTION kafka_conn
  TO KAFKA (BROKER '${testdrive.kafka-addr}', SECURITY PROTOCOL PLAINTEXT);

# Ingest the data in the reverse order but in separate partitions
$ kafka-ingest format=avro key-format=avro topic=data schema=${schema} key-schema=${key-schema} partition=2
{"a":3} {"before":null,"after":{"row":{"a":3,"b":1}},"source":{"file":"binlog","pos":3,"row":0,"snapshot":{"boolean":false}}, "op": "c"}

$ kafka-ingest format=avro key-format=avro topic=data schema=${schema} key-schema=${key-schema} partition=1
{"a":2} {"before":null,"after":{"row":{"a":2,"b":1}},"source":{"file":"binlog","pos":2,"row":0,"snapshot":{"boolean":false}}, "op": "c"}

$ kafka-ingest format=avro key-format=avro topic=data schema=${schema} key-schema=${key-schema} partition=0
{"a":1} {"before":null,"after":{"row":{"a":1,"b":1}},"source":{"file":"binlog","pos":1,"row":0,"snapshot":{"boolean":false}}, "op": "c"}

> CREATE SOURCE multipartition
  IN CLUSTER ${arg.single-replica-cluster}
  FROM KAFKA CONNECTION kafka_conn (TOPIC 'testdrive-data-${testdrive.seed}')

> CREATE TABLE multipartition_tbl FROM SOURCE multipartition (REFERENCE "testdrive-data-${testdrive.seed}")
  FORMAT AVRO USING CONFLUENT SCHEMA REGISTRY CONNECTION csr_conn
  ENVELOPE DEBEZIUM

> SELECT a, b FROM multipartition_tbl
a b
----
1 1
2 1
3 1
