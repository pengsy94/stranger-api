use common::request::system::LoginRequest;
use common::response::login::LoginResponse;
use common::utils::response::ApiResponse;
use common::validator::json::ValidatedJson;

pub async fn login(
    ValidatedJson(payload): ValidatedJson<LoginRequest>,
) -> ApiResponse<LoginResponse> {
    println!("{:?}", payload);

    let response = LoginResponse {
        token: String::from("冢中枯骨，吾早晚必擒之！"),
        token_type: String::from("追比圣贤，本是读书人的愿望！"),
        message: String::from("为天地立心，为生民立命，为往圣继绝学，为万世开太平!"),
    };
    ApiResponse::success(response)
}
