require "test_helper"

class Molecules::AuthHeading::ComponentTest < ViewComponent::TestCase
  test "renders the title and description" do
    heading = render_inline(Molecules::AuthHeading::Component.new(title: "Sign in", description: "Welcome back"))

    assert_equal "Sign in", heading.css("h1").text
    assert_equal "Welcome back", heading.css("p").text
  end

  test "renders the dim part of the title" do
    heading = render_inline(Molecules::AuthHeading::Component.new(title: "Welcome back", title_dim: "to your project."))

    assert_equal "Welcome back to your project.", heading.css("h1").text
    assert_equal "to your project.", heading.css("h1 span").text
  end

  test "omits the description when absent" do
    heading = render_inline(Molecules::AuthHeading::Component.new(title: "Sign in"))

    assert_empty heading.css("p")
  end
end
