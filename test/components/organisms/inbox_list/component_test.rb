require "test_helper"

class Organisms::InboxList::ComponentTest < ViewComponent::TestCase
  test "renders one row per item with a single action and the kind label" do
    items = Workspace::PlaceholderData.items(:inbox)
    list = render_inline(Organisms::InboxList::Component.new(items: items, preview: true, assign_href: "/x", repositories_href: "/ada/repositories"))

    assert_equal 3, list.css("li").size
    list.css("li").each { |row_| assert_equal 1, row_.css("a").size }
    assert_equal %w[Falló Merge Aprobar], list.css("li").map { |row_| row_.text[/Falló|Merge|Aprobar/] }
    assert_includes list.css("section").text, "Datos de ejemplo"
  end

  test "shows at most one flag and no flag on failed rows" do
    items = Workspace::PlaceholderData.items(:inbox)
    list = render_inline(Organisms::InboxList::Component.new(items: items, preview: true, assign_href: "/x", repositories_href: "/ada/repositories"))

    assert_includes list.css("li").to_a.find { |row_| row_.text.include?("Merge") }.text, "Modifica .project/"
    assert_not_includes list.css("li").first.text, "Tests fallaron"
  end

  test "omits the preview label for real data" do
    items = Workspace::PlaceholderData.items(:inbox)
    list = render_inline(Organisms::InboxList::Component.new(items: items, preview: false, assign_href: "/x", repositories_href: "/ada/repositories"))

    assert_not_includes list.text, "Datos de ejemplo"
  end

  test "renders the empty state with the assign link" do
    list = render_inline(Organisms::InboxList::Component.new(items: [], preview: false, assign_href: "/ada/repositories/meelting/issues", repositories_href: "/ada/repositories"))

    assert_equal "Nada te necesita", list.css("h3").text
    assert_equal "/ada/repositories/meelting/issues", list.css("a").first["href"]
    assert_equal "Asignar un issue", list.css("a").first.text.strip
  end
end
