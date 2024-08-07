# Copyright Materialize, Inc. and contributors. All rights reserved.
#
# Use of this software is governed by the Business Source License
# included in the LICENSE file at the root of this repository.
#
# As of the Change Date specified in that file, in accordance with
# the Business Source License, use of this software will be governed
# by the Apache License, Version 2.0.

$ kafka-create-topic topic=topic0 partitions=4

$ kafka-ingest key-format=bytes format=bytes key-terminator=: topic=topic0 repeat=1
1:1

> CREATE CONNECTION IF NOT EXISTS csr_conn TO CONFLUENT SCHEMA REGISTRY (
    URL '${testdrive.schema-registry-url}'
  );

> CREATE CONNECTION kafka_conn
  TO KAFKA (BROKER '${testdrive.kafka-addr}', SECURITY PROTOCOL PLAINTEXT);

> CREATE CLUSTER to_recreate SIZE '1', REPLICATION FACTOR 1;

> CREATE SOURCE source0
  IN CLUSTER to_recreate
  FROM KAFKA CONNECTION kafka_conn (TOPIC 'testdrive-topic0-${testdrive.seed}')
  KEY FORMAT TEXT
  VALUE FORMAT TEXT
  ENVELOPE UPSERT

> SELECT * FROM source0
key   text
----------
1     1

# Now recreate the topic with fewer partitions and observe the error

$ kafka-delete-topic-flaky topic=topic0

# Even though `kafka-delete-topic` ensures that the topic no longer exists in
# the broker metadata there is still work to be done asychnronously before it's
# truly gone that must complete before we attempt to recreate it. There is no
# way to observe this work completing so the only option left is sleeping for a
# while. This is the sad state of Kafka. If this test ever becomes flaky let's
# just delete it.
# See: https://github.com/confluentinc/confluent-kafka-python/issues/541
$ sleep-is-probably-flaky-i-have-justified-my-need-with-a-comment duration=2s

$ kafka-create-topic topic=topic0 partitions=2

! SELECT * FROM source0
contains:topic was recreated: partition count regressed from 4 to 2

# We can also detect that a topic got recreated by observing the high watermark regressing

$ kafka-create-topic topic=topic1 partitions=1

$ kafka-ingest format=bytes topic=topic1 repeat=1
1

> CREATE SOURCE source1
  IN CLUSTER to_recreate
  FROM KAFKA CONNECTION kafka_conn (TOPIC 'testdrive-topic1-${testdrive.seed}')
  FORMAT TEXT
  ENVELOPE NONE

> SELECT * FROM source1
text
----
1

# Now recreate the topic with the same number of partitions and observe the error

$ kafka-delete-topic-flaky topic=topic1

# Even though `kafka-delete-topic` ensures that the topic no longer exists in
# the broker metadata there is still work to be done asychnronously before it's
# truly gone that must complete before we attempt to recreate it. There is no
# way to observe this work completing so the only option left is sleeping for a
# while. This is the sad state of Kafka. If this test ever becomes flaky let's
# just delete it.
# See: https://github.com/confluentinc/confluent-kafka-python/issues/541
$ sleep-is-probably-flaky-i-have-justified-my-need-with-a-comment duration=2s

$ kafka-create-topic topic=topic1 partitions=1

! SELECT * FROM source1
contains:topic was recreated: high watermark of partition 0 regressed from 1 to 0

# Test a pathological topic recreation observed in the wild.
# See incidents-and-escalations#98.

# First we create a topic and successfully ingest some data.
$ kafka-create-topic topic=topic2 partitions=1
$ kafka-ingest format=bytes topic=topic2 repeat=100
one
> CREATE SOURCE source2
  IN CLUSTER to_recreate
  FROM KAFKA CONNECTION kafka_conn (TOPIC 'testdrive-topic2-${testdrive.seed}')
  FORMAT TEXT
  ENVELOPE NONE
> SELECT count(*) FROM source2
100

# Then we turn off the source cluster, so that we lose our record of what the
# high water mark used to be.
> ALTER CLUSTER to_recreate SET (REPLICATION FACTOR = 0)

# Then we delete the topic and recreate it...
# See comment above about needing to sleep after deleting Kafka topics.
$ kafka-delete-topic-flaky topic=topic2
$ sleep-is-probably-flaky-i-have-justified-my-need-with-a-comment duration=2s
$ kafka-create-topic topic=topic2 partitions=1

# ...crucially, with *fewer* offsets than we had previously.
$ kafka-ingest format=bytes topic=topic2 repeat=50
one

# Finally, we turn the source cluster back on. This would previously cause
# Materialize to panic because we'd attempt to regress the data shard's
# capability to offset 2 (the max offset in the new topic) when it was
# already at offset 3 (the max offset in the old topic).
> ALTER CLUSTER to_recreate SET (REPLICATION FACTOR = 1)

# Give the source a few seconds to reconnect to the Kafka partitions and
# possibly read bad data. This is what actually reproduces the panic we saw in
# incidents-and-escalations#98. Unfortunately there is no signal we can wait
# for, so the best we can do is sleep.
$ sleep-is-probably-flaky-i-have-justified-my-need-with-a-comment duration=10s

# Ensure the source reports the previous data.
> SELECT count(*) FROM source2
100

# Check whether the source is still lumbering along. Correctness has gone out
# the window here. Data in the new topic will be ignored up until the first new
# offset, at which point it will start being ingested. In this case, 7 and 8 are
# the two new data rows.
$ kafka-ingest format=bytes topic=topic2 repeat=53
one

> SELECT count(*) FROM source2
103

# Ensure we don't panic after we restart due to the above finished ingestions.
$ kafka-create-topic topic=good-topic

> CREATE SOURCE good_source
  IN CLUSTER to_recreate
  FROM KAFKA CONNECTION kafka_conn (TOPIC 'testdrive-good-topic-${testdrive.seed}')
  FORMAT TEXT
  ENVELOPE NONE

> ALTER CLUSTER to_recreate SET (REPLICATION FACTOR 0)
> ALTER CLUSTER to_recreate SET (REPLICATION FACTOR 1)

$ kafka-ingest format=bytes topic=good-topic repeat=1
1

> SELECT * FROM good_source
text
----
1

# TODO: why are these paused and not stalled with errors?
> SELECT name, status, error FROM mz_internal.mz_source_statuses WHERE type != 'progress'
name            status    error
-------------------------------
good_source     running   <null>
source0         paused    <null>
source1         paused    <null>
# Ideally source 2 would be permanently stalled because the topic was recreated,
# but we can't easily distingiush that situation from a temporary ingestion
# hiccup, and so at the moment we consider source2 to be fully healthy.
source2         running   <null>

# Testdrive expects all sources to end in a healthy state, so manufacture that
# by dropping sources.
> DROP CLUSTER to_recreate CASCADE;
