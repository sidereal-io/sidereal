-- Fixed credentials belong only to this disposable loopback fixture.
CREATE ROLE fixture_runtime LOGIN PASSWORD 'fixture_runtime';
CREATE ROLE fixture_test LOGIN CREATEDB PASSWORD 'fixture_test';
CREATE DATABASE fixture_runtime OWNER fixture_runtime;
GRANT CONNECT ON DATABASE fixture_control TO fixture_test;
