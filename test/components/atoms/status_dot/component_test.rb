require "test_helper"

class Atoms::StatusDot::ComponentTest < ViewComponent::TestCase
  test "renders an image with an accessible label" do
    dot = render_inline(Atoms::StatusDot::Component.new(tone: :bad, label: "Mirror fallando"))

    assert_equal "Mirror fallando", dot.css("span[role=img]").first["aria-label"]
    assert_includes dot.css("span").first["class"], "bg-danger-500"
  end
end
