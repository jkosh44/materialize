# Copyright Materialize, Inc. and contributors. All rights reserved.
#
# Use of this software is governed by the Business Source License
# included in the LICENSE file at the root of this repository.
#
# As of the Change Date specified in that file, in accordance with
# the Business Source License, use of this software will be governed
# by the Apache License, Version 2.0.
from textwrap import dedent

from materialize.checks.actions import Testdrive
from materialize.checks.checks import Check, externally_idempotent
from materialize.checks.executors import Executor
from materialize.mz_version import MzVersion


@externally_idempotent(False)
class JsonSource(Check):
    """Test CREATE SOURCE ... FORMAT JSON"""

    def _can_run(self, e: Executor) -> bool:
        return self.base_version >= MzVersion.parse_mz("v0.60.0-dev")

    def initialize(self) -> Testdrive:
        return Testdrive(
            dedent(
                """
                $ kafka-create-topic topic=format-json partitions=1

                $ kafka-ingest format=bytes key-format=bytes key-terminator=: topic=format-json
                "object":{"a":"b","c":"d"}

                > CREATE CLUSTER single_replica_cluster SIZE '1';

                >[version<11900] CREATE SOURCE format_jsonA
                  IN CLUSTER single_replica_cluster
                  FROM KAFKA CONNECTION kafka_conn (TOPIC 'testdrive-format-json-${testdrive.seed}')
                  KEY FORMAT JSON
                  VALUE FORMAT JSON
                  ENVELOPE UPSERT

                >[version>=11900] CREATE SOURCE format_jsonA_src
                  IN CLUSTER single_replica_cluster
                  FROM KAFKA CONNECTION kafka_conn (TOPIC 'testdrive-format-json-${testdrive.seed}')
                >[version>=11900] CREATE TABLE format_jsonA FROM SOURCE format_jsonA_src (REFERENCE "testdrive-format-json-${testdrive.seed}")
                  KEY FORMAT JSON
                  VALUE FORMAT JSON
                  ENVELOPE UPSERT

                >[version<11900] CREATE SOURCE format_jsonB
                  IN CLUSTER single_replica_cluster
                  FROM KAFKA CONNECTION kafka_conn (TOPIC 'testdrive-format-json-${testdrive.seed}')
                  KEY FORMAT JSON
                  VALUE FORMAT JSON
                  ENVELOPE UPSERT

                >[version>=11900] CREATE SOURCE format_jsonB_src
                  IN CLUSTER single_replica_cluster
                  FROM KAFKA CONNECTION kafka_conn (TOPIC 'testdrive-format-json-${testdrive.seed}')
                >[version>=11900] CREATE TABLE format_jsonB FROM SOURCE format_jsonB_src (REFERENCE "testdrive-format-json-${testdrive.seed}")
                  KEY FORMAT JSON
                  VALUE FORMAT JSON
                  ENVELOPE UPSERT
                """
            )
        )

    def manipulate(self) -> list[Testdrive]:
        return [
            Testdrive(dedent(s))
            for s in [
                """
                $ kafka-ingest format=bytes key-format=bytes key-terminator=: topic=format-json
                "float":1.23
                "str":"hello"
                """,
                """
                $ kafka-ingest format=bytes key-format=bytes key-terminator=: topic=format-json
                "array":[1,2,3]
                "int":1
                """,
            ]
        ]

    def validate(self) -> Testdrive:
        format_jsonB_source_name = (
            "format_jsonb"
            if self.base_version < MzVersion.parse_mz("v0.119.0")
            else "format_jsonb_src"
        )

        source_details = (
            " KEY FORMAT JSON VALUE FORMAT JSON ENVELOPE UPSERT"
            if self.base_version < MzVersion.parse_mz("v0.119.0")
            else ""
        )

        return Testdrive(
            dedent(
                """
                > SELECT * FROM format_jsonA ORDER BY key
                "\\"array\\"" [1,2,3]
                "\\"float\\"" 1.23
                "\\"int\\"" 1
                "\\"object\\"" "{\\"a\\":\\"b\\",\\"c\\":\\"d\\"}"
                "\\"str\\"" "\\"hello\\""

                > SELECT * FROM format_jsonB ORDER BY key
                "\\"array\\"" [1,2,3]
                "\\"float\\"" 1.23
                "\\"int\\"" 1
                "\\"object\\"" "{\\"a\\":\\"b\\",\\"c\\":\\"d\\"}"
                "\\"str\\"" "\\"hello\\""
                """
                + f"""
                > SHOW CREATE SOURCE {format_jsonB_source_name};
                materialize.public.{format_jsonB_source_name} "CREATE SOURCE \\"materialize\\".\\"public\\".\\"{format_jsonB_source_name}\\" IN CLUSTER \\"single_replica_cluster\\" FROM KAFKA CONNECTION \\"materialize\\".\\"public\\".\\"kafka_conn\\" (TOPIC = 'testdrive-format-json-${{testdrive.seed}}'){source_details} EXPOSE PROGRESS AS \\"materialize\\".\\"public\\".\\"{format_jsonB_source_name}_progress\\""
           """
            )
        )
