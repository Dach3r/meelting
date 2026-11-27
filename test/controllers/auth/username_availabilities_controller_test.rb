require "test_helper"

module Auth
  class UsernameAvailabilitiesControllerTest < ActionDispatch::IntegrationTest
    def check(username)
      get username_availability_path, params: { username: username }, headers: { "Turbo-Frame" => "username_availability" }
    end

    test "available username is confirmed without signing in" do
      check "grace"
      assert_response :success
      assert_select "turbo-frame#username_availability p[role=status]", text: I18n.t("auth.username_availabilities.show.available")
    end

    test "taken username is reported" do
      check users(:one).username
      assert_response :success
      assert_select "turbo-frame#username_availability p[role=alert]", text: I18n.t("activerecord.errors.models.user.attributes.username.taken")
    end

    test "invalid format is reported" do
      %w[Grace grace-hopper 1grace _grace].each do |username|
        check username
        assert_select "p[role=alert]", text: I18n.t("activerecord.errors.models.user.attributes.username.invalid")
      end
    end

    test "missing username is reported" do
      get username_availability_path
      assert_response :success
      assert_select "p[role=alert]"
    end

    test "non string username does not raise" do
      get username_availability_path, params: { username: { a: "b" } }
      assert_response :success
    end
  end
end
