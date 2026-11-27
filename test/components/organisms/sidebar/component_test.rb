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

    assert_equal "/sidebar_expansion", node.css("[data-controller=sidebar]").first["data-sidebar-url-value"]
  end

  test "offers a menu button for small screens that controls the navigation" do
    node = render_inline(Organisms::Sidebar::Component.new(workspace: workspace))

    button = node.css("button[data-action='sidebar#menu']").first
    assert_equal "false", button["aria-expanded"]
    assert_equal "sidebar-menu", button["aria-controls"]
    assert_equal 1, node.css("nav#sidebar-menu").size
  end

  test "holds the brand and stays fixed to the left on large screens" do
    node = render_inline(Organisms::Sidebar::Component.new(workspace: workspace))
    root = node.css("[data-controller=sidebar]").first

    assert_equal ApplicationHelper::CONFIG[:name], root.css("a[href='/']").first.text.strip
    %w[lg:fixed lg:inset-y-5 lg:left-0].each { |css_class_| assert_includes root["class"], css_class_ }
  end
end
