-- The minimal profile hosts the three logical stores as separate databases on one server.
CREATE DATABASE IF NOT EXISTS kurzhaar_raw;
CREATE DATABASE IF NOT EXISTS kurzhaar_api;
CREATE DATABASE IF NOT EXISTS kurzhaar_dashboard;
