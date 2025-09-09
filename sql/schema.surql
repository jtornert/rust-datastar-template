DEFINE ACCESS OVERWRITE users ON DATABASE TYPE RECORD
    SIGNUP ( fn::auth::signup($username, $password) )
    SIGNIN ( fn::auth::login($username, $password, $token_str) )
    DURATION FOR TOKEN 10s FOR SESSION 1h
;

DEFINE TABLE OVERWRITE user SCHEMAFULL PERMISSIONS FULL;
DEFINE FIELD OVERWRITE name ON user TYPE string ASSERT 2 <= string::len($value) AND string::len($value) <= 32;
DEFINE FIELD OVERWRITE _password ON user TYPE string ASSERT string::len($value) == 97 AND string::starts_with($value, "$argon2id") PERMISSIONS NONE;
DEFINE FIELD OVERWRITE registered_at ON user TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE user_name ON user FIELDS name UNIQUE;

DEFINE TABLE OVERWRITE session SCHEMAFULL PERMISSIONS FULL;
DEFINE FIELD OVERWRITE token ON session TYPE string;
DEFINE FIELD OVERWRITE nbf ON session TYPE datetime VALUE time::now();
DEFINE FIELD OVERWRITE ttl ON session TYPE datetime;
DEFINE FIELD OVERWRITE user ON session TYPE record<user>;
DEFINE INDEX OVERWRITE session_user ON session FIELDS user UNIQUE;
