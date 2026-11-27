require "test_helper"

class LandingTest < ActionDispatch::IntegrationTest
  test "guests see the landing header" do
    get root_path

    assert_response :success
    assert_select "header h1"
  end

  test "landing uses the global guest navbar" do
    get root_path

    assert_select "nav", count: 1
    assert_select "body > nav"
  end

  test "landing is available in english" do
    get root_path, headers: { "Accept-Language" => "en" }

    I18n.with_locale(:en) do
      assert_equal "Meelting: a safe berth for every agent run", I18n.t("landing.title", name: ApplicationHelper::CONFIG[:name])
    end
  end

  test "signed in users get the app home instead of the landing" do
    sign_in_as users(:one)
    get root_path

    assert_equal "home", @controller.controller_name
    assert_select "header h1", false
  end

  test "layout preloads the Inter font and sets the brand title" do
    get root_path

    assert_select "link[rel=preload][as=font][type='font/woff2']"
    assert_select "title", /Meelting/
  end

  test "sign in page keeps the standard navbar" do
    get new_session_path

    assert_select "body > nav"
  end
end
