require "test_helper"

class Organisms::Footer::ComponentTest < ViewComponent::TestCase
  test "renders brand, tagline and license" do
    footer = render_inline(Organisms::Footer::Component.new)

    assert_equal ApplicationHelper::CONFIG[:name], footer.css("a[href='/']").text.strip
    assert_includes footer.text, "AGPL-3.0"
  end
end
