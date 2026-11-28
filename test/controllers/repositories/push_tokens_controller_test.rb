require "test_helper"

class Repositories::PushTokensControllerTest < ActionDispatch::IntegrationTest
  setup { sign_in_as users(:one) }

  test "requires authentication" do
    sign_out

    assert_no_difference "AccessToken.count" do
      post repository_push_token_path("ada", "meelting")
    end
    assert_redirected_to new_session_path
  end

  test "creates a git token and shows it once inside the push commands" do
    assert_difference -> { users(:one).access_tokens.count } do
      post repository_push_token_path("ada", "meelting"), as: :turbo_stream
    end

    token = users(:one).access_tokens.order(:created_at).last
    assert_equal %w[git_read git_write], token.scopes
    assert_equal "Push de Meelting", token.name
    assert_select "turbo-stream[action=replace][target=push_commands]"
    assert_includes response.body, "git remote add origin http://www.example.com/ada/meelting.git"
    plain_token = response.body[/(#{AccessToken::PREFIX}[\w-]+)/, 1]
    assert_equal token, AccessToken.authenticate(plain_token, scope: :git_write)
    assert_no_match(/:#{plain_token}@/, response.body)
  end

  test "falls back to the access tokens page without Turbo" do
    post repository_push_token_path("ada", "meelting")

    assert_redirected_to access_tokens_path
    assert flash[:plain_token].start_with?(AccessToken::PREFIX)
  end

  test "does not create tokens for another user's repository" do
    assert_no_difference "AccessToken.count" do
      post repository_push_token_path("alan", "notes"), as: :turbo_stream
    end
    assert_response :not_found
  end
end
