require "test_helper"

class Molecules::SidebarRepository::ComponentTest < ViewComponent::TestCase
  def repository(real: true, mirror: :ok)
    Workspace::RepositoryEntry.new(name: "Meelting", slug: "meelting", username: "ada", mirror: mirror, real: real)
  end

  test "collapsed renders the button with aria-expanded false and a hidden panel" do
    node = render_inline(Molecules::SidebarRepository::Component.new(repository: repository, expanded: false))

    assert_equal "false", node.css("button").first["aria-expanded"]
    assert_equal "sidebar-repository-meelting", node.css("button").first["aria-controls"]
    assert node.css("#sidebar-repository-meelting").first.key?("hidden")
  end

  test "expanded shows the five sections" do
    node = render_inline(Molecules::SidebarRepository::Component.new(repository: repository, expanded: true))

    assert_equal "true", node.css("button").first["aria-expanded"]
    assert_not node.css("#sidebar-repository-meelting").first.key?("hidden")
    assert_equal ["Código", "Issues", "Pull requests", "Runs", "Memoria"], node.css("#sidebar-repository-meelting a").map(&:text)
  end

  test "labels the mirror dot with its state" do
    { ok: "Mirror al día", failing: "Mirror fallando", none: "Sin mirror" }.each do |mirror, label|
      node = render_inline(Molecules::SidebarRepository::Component.new(repository: repository(mirror: mirror), expanded: false))

      assert_equal label, node.css("[role=img]").first["aria-label"]
    end
  end

  test "placeholder repositories link to the repositories list" do
    node = render_inline(Molecules::SidebarRepository::Component.new(repository: repository(real: false), expanded: true))

    assert_equal ["/repositories"], node.css("a").map { |link_| link_["href"] }.uniq
  end
end
