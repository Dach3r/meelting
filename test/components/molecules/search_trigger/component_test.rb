require "test_helper"

class Molecules::SearchTrigger::ComponentTest < ViewComponent::TestCase
  test "renders an accessible button with the shortcut hint" do
    button = render_inline(Molecules::SearchTrigger::Component.new).css("button")

    assert_equal "button", button.attr("type").value
    assert button.attr("aria-label").value.present?
    assert_equal "⌘K", button.css("kbd").text
  end
end
