require "test_helper"

module Auth
  class RegistrationsControllerTest < ActionDispatch::IntegrationTest
    def valid_params(overrides = {})
      { user: { username: "grace", first_name: "Grace", last_name: "Hopper", email_address: "new@example.com", password: "password123" }.merge(overrides) }
    end

    test "new renders sign up form" do
      get new_registration_path
      assert_response :success
    end

    test "create with valid params creates user and signs in" do
      assert_difference ["User.count", "Session.count"], 1 do
        post registration_path, params: valid_params
      end
      assert_redirected_to root_url
      assert cookies[:session_id].present?
      assert User.exists?(email_address: "new@example.com")
    end

    test "create stores names" do
      post registration_path, params: valid_params
      user = User.find_by!(email_address: "new@example.com")
      assert_equal "Grace", user.first_name
      assert_equal "Hopper", user.last_name
    end

    test "create without first name fails" do
      assert_no_difference "User.count" do
        post registration_path, params: valid_params(first_name: "")
      end
      assert_response :unprocessable_entity
    end

    test "create without last name fails" do
      assert_no_difference "User.count" do
        post registration_path, params: valid_params(last_name: "")
      end
      assert_response :unprocessable_entity
    end

    test "create normalizes email" do
      post registration_path, params: valid_params(email_address: "  NEW@Example.com ")
      assert User.exists?(email_address: "new@example.com")
    end

    test "create with duplicate email fails" do
      assert_no_difference "User.count" do
        post registration_path, params: valid_params(email_address: users(:one).email_address)
      end
      assert_response :unprocessable_entity
    end

    test "create with invalid email fails" do
      assert_no_difference "User.count" do
        post registration_path, params: valid_params(email_address: "bad")
      end
      assert_response :unprocessable_entity
    end

    test "create with short password fails" do
      assert_no_difference "User.count" do
        post registration_path, params: valid_params(password: "short")
      end
      assert_response :unprocessable_entity
    end

    test "failed create does not start session" do
      assert_no_difference "Session.count" do
        post registration_path, params: valid_params(password: "short")
      end
    end
  end
end
