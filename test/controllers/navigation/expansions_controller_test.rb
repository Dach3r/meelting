require "test_helper"

class Navigation::ExpansionsControllerTest < ActionDispatch::IntegrationTest
  test "requires authentication" do
    patch sidebar_expansion_path, params: { repository_slug: "meelting" }

    assert_redirected_to new_session_path
  end

  test "remembers the expanded repository and renders it expanded on the next load" do
    sign_in_as users(:one)
    patch sidebar_expansion_path, params: { repository_slug: "meelting" }

    assert_response :no_content
    get root_path
    assert_select "button[aria-expanded=true]", /Meelting/
    assert_select "#sidebar-repository-meelting:not([hidden])"
  end

  test "renders collapsed when nothing was expanded" do
    sign_in_as users(:one)
    get root_path

    assert_select "button[aria-expanded=true]", count: 0
    assert_select "#sidebar-repository-meelting[hidden]"
  end

  test "collapsing clears the remembered repository" do
    sign_in_as users(:one)
    patch sidebar_expansion_path, params: { repository_slug: "meelting" }
    patch sidebar_expansion_path, params: { repository_slug: "" }
    get root_path

    assert_select "button[aria-expanded=true]", count: 0
  end

  test "ignores slugs that are not valid" do
    sign_in_as users(:one)
    patch sidebar_expansion_path, params: { repository_slug: "../etc" }
    get root_path

    assert_select "button[aria-expanded=true]", count: 0
  end
end
