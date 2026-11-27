require "test_helper"

class LandingTest < ActionDispatch::IntegrationTest
  test "guests see the landing hero" do
    get root_path

    assert_response :success
    assert_select "main section h1"
  end

  test "landing uses its own light header" do
    get root_path

    assert_select "body > header nav", count: 1
    assert_select "body > header nav a[href=?]", new_session_path
    assert_select "body > nav", false
  end

  test "landing is available in english" do
    get root_path, headers: { "Accept-Language" => "en" }

    I18n.with_locale(:en) do
      assert_equal "Meelting: where humans and AI agents build software together", I18n.t("landing.title", name: ApplicationHelper::CONFIG[:name])
    end
  end

  test "signed in users get the home instead of the landing" do
    sign_in_as users(:one)
    get root_path

    assert_equal "home", @controller.controller_name
    assert_select "main section h1", false
  end

  test "layout preloads the Inter font and sets the brand title" do
    get root_path

    assert_select "link[rel=preload][as=font][type='font/woff2']"
    assert_select "title", /Meelting/
  end
end
