require "test_helper"

class AuthLayoutTest < ActionDispatch::IntegrationTest
  test "auth screens share the auth layout without the navbar" do
    [new_session_path, new_registration_path, new_password_path].each do |path|
      get path

      assert_response :success
      assert_select "body > nav", count: 0
    end
  end

  test "flash messages render inside the layout" do
    post session_path, params: { username: "nobody", password: "wrong" }
    follow_redirect!

    assert_select "#alert[role=alert]"
  end

  test "sign in page sets its own title" do
    get new_session_path

    assert_select "title", I18n.t("auth.sessions.new.title")
  end
end
