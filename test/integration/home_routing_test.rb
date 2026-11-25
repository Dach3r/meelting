require "test_helper"

class HomeRoutingTest < ActionDispatch::IntegrationTest
  test "root without session renders sessions#new" do
    get root_path
    assert_response :success
    assert_select "h1", "Sign in"
  end

  test "root with session renders home#index" do
    sign_in_as users(:one)
    get root_path
    assert_response :success
    assert_select "h1", false
    assert_equal "home", @controller.controller_name
  end

  test "root with stale session cookie falls back to sign in" do
    sign_in_as users(:one)
    Session.destroy_all
    get root_path
    assert_select "h1", "Sign in"
  end

  test "root after sign out renders sign in" do
    sign_in_as users(:one)
    sign_out
    get root_path
    assert_select "h1", "Sign in"
  end

  test "home greets the user and offers sign out" do
    sign_in_as users(:one)
    get root_path
    assert_select "p", "¡Hola Ada Lovelace!"
    assert_select "form[action=?] button", session_path, "Cerrar sesión"
  end
end
