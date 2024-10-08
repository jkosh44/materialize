# Copyright Materialize, Inc. and contributors. All rights reserved.
#
# Use of this software is governed by the Business Source License
# included in the LICENSE file at the root of this repository.
#
# As of the Change Date specified in that file, in accordance with
# the Business Source License, use of this software will be governed
# by the Apache License, Version 2.0.

#
# Test reg* data types
#

> CREATE SECRET pgpass AS 'postgres'
> CREATE CONNECTION pg TO POSTGRES (
    HOST postgres,
    DATABASE postgres,
    USER postgres,
    PASSWORD SECRET pgpass
  )


# Insert data pre-snapshot
$ postgres-execute connection=postgres://postgres:postgres@postgres
ALTER USER postgres WITH replication;
DROP SCHEMA IF EXISTS public CASCADE;
DROP PUBLICATION IF EXISTS mz_source;

CREATE SCHEMA public;

CREATE TABLE regtype_table (f1 REGTYPE);
ALTER TABLE regtype_table REPLICA IDENTITY FULL;
INSERT INTO regtype_table VALUES (1::regtype),(0::regtype);

CREATE TABLE regclass_table (f1 REGCLASS);
ALTER TABLE regclass_table REPLICA IDENTITY FULL;
INSERT INTO regclass_table VALUES (1::regclass),(0::regclass);

CREATE TABLE regproc_table (f1 REGPROC);
ALTER TABLE regproc_table REPLICA IDENTITY FULL;
INSERT INTO regproc_table VALUES (1::regproc),(2::regproc);

CREATE PUBLICATION mz_source FOR ALL TABLES;

> CREATE SOURCE mz_source FROM POSTGRES CONNECTION pg (PUBLICATION 'mz_source');

! CREATE TABLE regtype_table FROM SOURCE mz_source (REFERENCE regtype_table);
contains:table regtype_table contains column f1 of type regtype which Materialize cannot currently ingest

! CREATE TABLE regclass_table FROM SOURCE mz_source (REFERENCE regclass_table);
contains:table regclass_table contains column f1 of type regclass which Materialize cannot currently ingest

! CREATE TABLE regproc_table FROM SOURCE mz_source (REFERENCE regproc_table);
contains:table regproc_table contains column f1 of type regproc which Materialize cannot currently ingest

> CREATE TABLE regproc_table FROM SOURCE mz_source (REFERENCE regproc_table) WITH (TEXT COLUMNS = (f1));

> SELECT * FROM regproc_table;
1
2
