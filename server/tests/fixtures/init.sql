-- Fixed credentials belong only to this disposable loopback fixture.
CREATE ROLE fixture_runtime LOGIN PASSWORD 'fixture_runtime';
CREATE ROLE fixture_demo LOGIN PASSWORD 'fixture_demo';
CREATE ROLE fixture_test LOGIN CREATEDB PASSWORD 'fixture_test';
CREATE DATABASE fixture_runtime OWNER fixture_runtime;
CREATE DATABASE fixture_demo OWNER fixture_demo;
GRANT CONNECT ON DATABASE fixture_control TO fixture_test;
