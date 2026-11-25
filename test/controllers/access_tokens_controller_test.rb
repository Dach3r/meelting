require "test_helper"

class AccessTokensControllerTest < ActionDispatch::IntegrationTest
  setup do
    @user = users(:one)
    sign_in_as @user
  end

  test "requires authentication" do
    sign_out
    get access_tokens_path
    assert_redirected_to new_session_path
  end

  test "index lists only the current user's tokens" do
    get access_tokens_path
    assert_response :success
    assert_select "##{dom_id(access_tokens(:git))}"
    assert_select "##{dom_id(access_tokens(:other_user))}", count: 0
  end

  test "new renders form" do
    get new_access_token_path
    assert_response :success
  end

  test "create stores the token and shows it once" do
    assert_difference "@user.access_tokens.count", 1 do
      post access_tokens_path, params: { access_token: { name: "CI", scopes: %w[api] } }
    end
    assert_redirected_to access_tokens_path

    follow_redirect!
    assert_select "#plain_token code", text: /\Amlt_/

    get access_tokens_path
    assert_select "#plain_token", count: 0
  end

  test "create with invalid params re-renders the form" do
    assert_no_difference "AccessToken.count" do
      post access_tokens_path, params: { access_token: { name: "", scopes: [""] } }
    end
    assert_response :unprocessable_entity
  end

  test "destroy revokes the token" do
    token = access_tokens(:git)
    delete access_token_path(token)
    assert_redirected_to access_tokens_path
    assert token.reload.revoked?
  end

  test "destroy cannot revoke another user's token" do
    other = access_tokens(:other_user)
    delete access_token_path(other)
    assert_response :not_found
    assert_not other.reload.revoked?
  end
end
