require "test_helper"

class Molecules::UserMenu::ComponentTest < ViewComponent::TestCase
  test "shows initials, full name and the account links" do
    menu = render_inline(Molecules::UserMenu::Component.new(user: users(:one)))

    assert_includes menu.css("summary").text, "AL"
    assert_includes menu.css("summary").text, "Ada Lovelace"
    assert_equal 1, menu.css("a[href='/repositories']").size
    assert_equal 1, menu.css("a[href='/access_tokens']").size
    assert_equal 1, menu.css("form[action='/session'] button").size
  end
end
