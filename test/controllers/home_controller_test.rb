require "test_helper"

class HomeControllerTest < ActionDispatch::IntegrationTest
  test "requires authentication through the root constraint" do
    get root_path

    assert_equal "landing", @controller.controller_name
  end

  test "show renders the inbox with the first decision selected" do
    sign_in_as users(:one)
    get root_path

    assert_response :success
    assert_select "h1.sr-only", "Inicio"
    assert_select "section#decisions a", 4
    assert_select "a[aria-current=true]", /Configurable push mirror/
    assert_select "h2", /Configurable push mirror to GitHub/
  end

  test "show selects the decision from the item param" do
    sign_in_as users(:one)
    get root_path(item: "pr_9")

    assert_select "a[aria-current=true]", /Push and clone over HTTPS/
    assert_select "span", /Conflictos con main/
  end

  test "navbar shows the inbox counters on every signed in page" do
    sign_in_as users(:one)
    get repositories_path

    assert_select "nav a[href=?]", root_path(anchor: "decisions"), /2\s+esperan aprobación/
    assert_select "nav a[href=?]", root_path(anchor: "working"), /1\s+en curso/
    assert_select "nav summary", /AL\s+ada/
  end
end
