require "test_helper"

class NavbarTest < ActionDispatch::IntegrationTest
  test "shows guest links without a session" do
    get unauthenticated_root_path

    assert_select "nav a[href=?]", new_session_path
    assert_select "nav form[action=?]", session_path, count: 0
  end

  test "shows user links with a session" do
    sign_in_as users(:one)
    get repositories_path

    assert_select "nav a[href=?]", access_tokens_path
    assert_select "nav form[action=?]", session_path
  end
end
