CREATE DATABASE vidcall;

-- user
CREATE TABLE users (
    id uuid unique,
    username VARCHAR(100) unique,
    email varchar(254) unique ,
    password text,
    salt text
);
CREATE INDEX users_username ON users (username);
CREATE INDEX users_email ON users (email);

SELECT * FROM users;

DROP TABLE  users;
DELETE FROM users WHERE username = 'arfan';

-- auth
CREATE TABLE auth (
    userid uuid unique,
    appid varch ar(100)
);