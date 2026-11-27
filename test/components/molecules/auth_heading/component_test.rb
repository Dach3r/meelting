require "test_helper"

class Molecules::AuthHeading::ComponentTest < ViewComponent::TestCase
  test "renders the title and description" do
    heading = render_inline(Molecules::AuthHeading::Component.new(title: "Sign in", description: "Welcome back"))

    assert_equal "Sign in", heading.css("h1").text
    assert_equal "Welcome back", heading.css("p").text
  end

  test "omits the description when absent" do
    heading = render_inline(Molecules::AuthHeading::Component.new(title: "Sign in"))

    assert_empty heading.css("p")
  end
end
