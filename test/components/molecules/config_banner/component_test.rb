require "test_helper"

class Molecules::ConfigBanner::ComponentTest < ViewComponent::TestCase
  test "renders an alert with the problem and a fix link" do
    problem = Workspace::Problem.new(kind: :mirror_failing, repository: "veta")
    banner = render_inline(Molecules::ConfigBanner::Component.new(problem: problem))

    assert_includes banner.css("[role=alert]").text, "El mirror de GitHub de veta está fallando."
    assert_equal "/settings", banner.css("a").first["href"]
  end
end
