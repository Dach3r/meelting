require "test_helper"

module Auth
  class PasswordsControllerTest < ActionDispatch::IntegrationTest
    include ActionMailer::TestHelper

    setup { @user = users(:one) }

    test "new renders" do
      get new_password_path
      assert_response :success
    end

    test "create for existing user enqueues reset email" do
      assert_enqueued_email_with PasswordsMailer, :reset, args: [@user] do
        post passwords_path, params: { email_address: @user.email_address }
      end
      assert_redirected_to new_session_path
    end

    test "create for unknown user sends nothing but redirects identically" do
      assert_no_enqueued_emails do
        post passwords_path, params: { email_address: "missing@example.com" }
      end
      assert_redirected_to new_session_path
      assert_match "if user with that email", flash[:notice]
    end

    test "edit with valid token renders" do
      get edit_password_path(@user.password_reset_token)
      assert_response :success
    end

    test "edit with invalid token redirects" do
      get edit_password_path("invalid-token")
      assert_redirected_to new_password_path
      assert_match "invalid or has expired", flash[:alert]
    end

    test "update with valid token changes password" do
      assert_changes -> { @user.reload.password_digest } do
        put password_path(@user.password_reset_token), params: { password: "newpassword123", password_confirmation: "newpassword123" }
      end
      assert_redirected_to new_session_path
      assert User.authenticate_by(email_address: @user.email_address, password: "newpassword123")
    end

    test "update destroys existing sessions" do
      @user.sessions.create!
      assert_difference "Session.count", -1 do
        put password_path(@user.password_reset_token), params: { password: "newpassword123", password_confirmation: "newpassword123" }
      end
    end

    test "update with mismatched passwords does not change password" do
      token = @user.password_reset_token
      assert_no_changes -> { @user.reload.password_digest } do
        put password_path(token), params: { password: "newpassword123", password_confirmation: "other12345" }
      end
      assert_redirected_to edit_password_path(token)
      assert_match "did not match", flash[:alert]
    end

    test "update with invalid token redirects" do
      put password_path("invalid-token"), params: { password: "newpassword123", password_confirmation: "newpassword123" }
      assert_redirected_to new_password_path
    end

    test "token is invalid after password changes" do
      token = @user.password_reset_token
      put password_path(token), params: { password: "newpassword123", password_confirmation: "newpassword123" }
      get edit_password_path(token)
      assert_redirected_to new_password_path
    end
  end
end
