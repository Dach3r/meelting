require "test_helper"

class Organisms::Sidebar::ComponentTest < ViewComponent::TestCase
  def workspace(scenario = nil)
    Workspace::Build.call(user: users(:one), scenario: scenario)
  end

  test "has a single expanded repository at most" do
    node = render_inline(Organisms::Sidebar::Component.new(workspace: workspace(:empty), expanded_slug: "meelting"))

    assert_operator node.css("button[aria-expanded=true]").size, :<=, 1
  end

  test "ignores a remembered slug that is not in the list" do
    node = render_inline(Organisms::Sidebar::Component.new(workspace: workspace, expanded_slug: "gone"))

    assert_empty node.css("button[aria-expanded=true]")
  end

  test "shows no counters when there is nothing to count" do
    node = render_inline(Organisms::Sidebar::Component.new(workspace: workspace))

    assert_empty node.css("[aria-label$=pendientes]")
  end

  test "wires the Stimulus controller with the persistence url" do
    node = render_inline(Organisms::Sidebar::Component.new(workspace: workspace))

    assert_equal "sidebar", node.css("nav").first["data-controller"]
    assert_equal "/sidebar_expansion", node.css("nav").first["data-sidebar-url-value"]
  end
end
