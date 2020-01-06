create table CUSTOMERS (
        id INT PRIMARY KEY,
        first_name VARCHAR(50),
        last_name VARCHAR(50),
        email VARCHAR(50),
        gender VARCHAR(50),
	      club_status VARCHAR(8),
        comments VARCHAR(90),
        create_ts timestamp DEFAULT CURRENT_TIMESTAMP ,
        update_ts timestamp DEFAULT CURRENT_TIMESTAMP
);