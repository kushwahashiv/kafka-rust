CREATE TABLE balance (
 id TEXT NOT NULL PRIMARY KEY,
 account_no TEXT,
 token TEXT,
 account_type TEXT,
 amount REAL,
 limit REAL,
 updated_at TIMESTAMP,
 created_at TIMESTAMP NOT NULL
);
create table confirmed_account(
  id TEXT NOT NULL PRIMARY KEY,
  account_no TEXT,
  token TEXT,
  account_type TEXT,
  reason TEXT,
  created_at -> Timestamp,
  updated_at -> Timestamp
  );

  create table confirmed_transaction (
  id TEXT NOT NULL PRIMARY KEY,
  reason TEXT,
  created_at -> Timestamp,
  );