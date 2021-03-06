CREATE TABLE transactions (
 id TEXT NOT NULL PRIMARY KEY,
 account_no TEXT,
 amount REAL,
 new_balance REAL,
 account_type TEXT,
 changed_by TEXT,
 from_to TEXT,
 direction TEXT,
 description TEXT,
 created_at TIMESTAMP
);

create table account(
  id TEXT NOT NULL PRIMARY KEY,
  username TEXT,
  password TEXT,
  created_at TIMESTAMP
);
