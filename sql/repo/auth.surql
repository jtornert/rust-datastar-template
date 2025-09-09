DEFINE FUNCTION OVERWRITE fn::auth::signup($username: string, $password: string) {
    RETURN CREATE ONLY user SET name = $username, _password = crypto::argon2::generate($password);
} PERMISSIONS WHERE !type::is::record($auth, "user");

DEFINE FUNCTION OVERWRITE fn::auth::login($username: option<string>, $password: option<string>, $token_str: option<string>) {
    IF $username != NONE AND $password != NONE {
        RETURN SELECT id FROM ONLY user WHERE name = $username AND crypto::argon2::compare(_password, $password) LIMIT 1;
    };

    IF $token_str != NONE {
        RETURN SELECT user as id, ttl FROM ONLY session WHERE token = $token_str AND ttl > time::now() LIMIT 1;
    };
} PERMISSIONS WHERE !type::is::record($auth, "user");

DEFINE FUNCTION OVERWRITE fn::auth::create_session($user: record<user>) {
    RETURN UPSERT ONLY session SET token = rand::uuid::v4(), user = $user, ttl = time::now() + 1h WHERE user = $user;
};

DEFINE FUNCTION OVERWRITE fn::auth::invalidate_session($token_str: string) {
    RETURN UPDATE ONLY session SET ttl = time::now() WHERE token = $token_str;
};