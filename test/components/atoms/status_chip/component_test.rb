require "test_helper"

class Atoms::StatusChip::ComponentTest < ViewComponent::TestCase
  test "renders the content with the tone classes" do
    chip = render_inline(Atoms::StatusChip::Component.new(tone: :wait)) { "Approval" }.css("span")

    assert_equal "Approval", chip.text.strip
    assert_includes chip.attr("class").value, "bg-sun-400"
  end

  test "renders an icon before the content" do
    chip = render_inline(Atoms::StatusChip::Component.new(tone: :ok, icon: :check)) { "Mergeable" }

    assert_equal 1, chip.css("span svg").size
  end

  test "raises on an unknown tone" do
    assert_raises(KeyError) { render_inline(Atoms::StatusChip::Component.new(tone: :nope)) { "x" } }
  end
end
