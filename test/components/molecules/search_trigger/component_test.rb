require "test_helper"

class Molecules::SearchTrigger::ComponentTest < ViewComponent::TestCase
  test "renders a full width accessible button without a shortcut hint" do
    button = render_inline(Molecules::SearchTrigger::Component.new).css("button")

    assert_equal "button", button.attr("type").value
    assert button.attr("aria-label").value.present?
    assert_includes button.attr("class").value, "flex-[1_1_280px]"
    assert_empty button.css("kbd")
  end
end
