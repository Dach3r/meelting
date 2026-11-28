require "test_helper"

class Molecules::UserMenu::ComponentTest < ViewComponent::TestCase
  test "shows the initial, full name, settings and sign out" do
    menu = render_inline(Molecules::UserMenu::Component.new(user: users(:one)))

    assert_equal "A", menu.css("summary span[aria-hidden]").text.strip
    assert_includes menu.css("summary").text, "Ada Lovelace"
    assert_equal 1, menu.css("a").size
    assert_equal 1, menu.css("a[href='/settings']").size
    assert_equal 1, menu.css("form[action='/session'] button").size
    assert_equal 1, menu.css("details[data-controller=dropdown]").size
  end
end
