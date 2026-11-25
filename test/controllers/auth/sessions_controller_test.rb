require "test_helper"

module Auth
  class SessionsControllerTest < ActionDispatch::IntegrationTest
    setup { @user = users(:one) }

    test "new renders sign in form" do
      get new_session_path
      assert_response :success
    end

    test "create with valid credentials starts session and redirects to home" do
      assert_difference "Session.count", 1 do
        post session_path, params: { email_address: @user.email_address, password: "password123" }
      end
      assert_redirected_to root_url
      assert cookies[:session_id].present?
    end

    test "create with wrong password does not start session" do
      assert_no_difference "Session.count" do
        post session_path, params: { email_address: @user.email_address, password: "wrong" }
      end
      assert_redirected_to new_session_path
      assert_nil cookies[:session_id].presence
    end

    test "create with unknown email does not start session" do
      assert_no_difference "Session.count" do
        post session_path, params: { email_address: "nobody@example.com", password: "password123" }
      end
      assert_redirected_to new_session_path
    end

    test "destroy terminates session" do
      sign_in_as @user
      assert_difference "Session.count", -1 do
        delete session_path
      end
      assert_redirected_to new_session_path
      assert_empty cookies[:session_id].to_s
    end
  end
end
