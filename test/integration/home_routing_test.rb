require "test_helper"

class HomeRoutingTest < ActionDispatch::IntegrationTest
  test "root without session renders the landing" do
    get root_path
    assert_response :success
    assert_equal "landing", @controller.controller_name
    assert_select "h1", /La memoria de tu proyecto/
  end

  test "root with session renders home#index" do
    sign_in_as users(:one)
    get root_path
    assert_response :success
    assert_select "h1", false
    assert_equal "home", @controller.controller_name
  end

  test "root with stale session cookie falls back to the landing" do
    sign_in_as users(:one)
    Session.destroy_all
    get root_path
    assert_equal "landing", @controller.controller_name
  end

  test "root after sign out renders the landing" do
    sign_in_as users(:one)
    sign_out
    get root_path
    assert_equal "landing", @controller.controller_name
  end

  test "home greets the user and offers sign out" do
    sign_in_as users(:one)
    get root_path
    assert_select "p", "¡Hola Ada Lovelace!"
    assert_select "form[action=?] button", session_path, "Cerrar sesión"
  end
end
