require "test_helper"

class Atoms::Icon::ComponentTest < ViewComponent::TestCase
  test "renders a lucide svg with extra classes" do
    svg = render_inline(Atoms::Icon::Component.new(name: :shield_check, class: "size-6")).css("svg")

    assert_equal "size-6", svg.attr("class").value
    assert_equal "true", svg.attr("aria-hidden").value
  end
end
