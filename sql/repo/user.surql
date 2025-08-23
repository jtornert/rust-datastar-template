DEFINE FUNCTION OVERWRITE fn::user::get($user: record<user>) {
    RETURN SELECT * FROM $user;
};