require "test_helper"

class Organisms::Sidebar::ComponentTest < ViewComponent::TestCase
  def workspace(scenario = nil)
    Workspace::Build.call(user: users(:one), scenario: scenario)
  end

  test "has a single expanded repository at most" do
    node = render_inline(Organisms::Sidebar::Component.new(workspace: workspace(:empty), user: users(:one), expanded_slug: "meelting"))

    assert_operator node.css("button[aria-expanded=true]").size, :<=, 1
  end

  test "ignores a remembered slug that is not in the list" do
    node = render_inline(Organisms::Sidebar::Component.new(workspace: workspace, user: users(:one), expanded_slug: "gone"))

    assert_empty node.css("button[aria-expanded=true]")
  end

  test "shows no counters when there is nothing to count" do
    node = render_inline(Organisms::Sidebar::Component.new(workspace: workspace, user: users(:one)))

    assert_empty node.css("[aria-label$=pendientes]")
  end

  test "wires the Stimulus controller with the persistence url" do
    node = render_inline(Organisms::Sidebar::Component.new(workspace: workspace, user: users(:one)))

    assert_equal "/sidebar_expansion", node.css("[data-controller=sidebar]").first["data-sidebar-url-value"]
  end

  test "offers a menu button for small screens that controls the navigation" do
    node = render_inline(Organisms::Sidebar::Component.new(workspace: workspace, user: users(:one)))

    button = node.css("button[data-action='sidebar#menu']").first
    assert_equal "false", button["aria-expanded"]
    assert_equal "sidebar-menu", button["aria-controls"]
    assert_equal 1, node.css("nav#sidebar-menu").size
  end

  test "holds the brand and stays fixed to the left on large screens" do
    node = render_inline(Organisms::Sidebar::Component.new(workspace: workspace, user: users(:one)))
    root = node.css("[data-controller=sidebar]").first

    assert_equal ApplicationHelper::APP_NAME, root.css("#sidebar-menu a[href='/']").first["aria-label"]
    %w[lg:fixed lg:inset-y-0 lg:left-0 lg:w-23 lg:group-data-sidebar-expanded/shell:w-66].each { |css_class_| assert_includes root["class"], css_class_ }
  end

  test "renders an icon rail that expands and an expanded menu that collapses" do
    node = render_inline(Organisms::Sidebar::Component.new(workspace: workspace, user: users(:one)))

    rail = node.css("nav:not(#sidebar-menu)").first
    assert_equal [I18n.t("sidebar.inbox"), I18n.t("sidebar.runs"), I18n.t("sidebar.repositories")], rail.css("a[aria-label]").map { |link_| link_["aria-label"] }.drop(1)
    assert_equal I18n.t("sidebar.expand"), rail.css("button[data-action='sidebar#expand']").first["aria-label"]
    assert_equal 1, node.css("#sidebar-menu button[data-action='sidebar#collapse']").size
    assert_equal ApplicationHelper::SIDEBAR_COOKIE, node.css("[data-controller=sidebar]").first["data-sidebar-cookie-value"]
  end

  test "holds the user menu in the rail and in the expanded menu" do
    node = render_inline(Organisms::Sidebar::Component.new(workspace: workspace, user: users(:one)))

    rail_menu = node.css("nav:not(#sidebar-menu) details summary").first
    assert_equal "A", rail_menu.text.strip
    assert_includes node.css("#sidebar-menu details summary").text, "Ada Lovelace"
    assert_equal 2, node.css("details form[action='/session'] button").size
  end
end
