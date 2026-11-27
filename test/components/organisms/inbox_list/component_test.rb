require "test_helper"

class Organisms::InboxList::ComponentTest < ViewComponent::TestCase
  test "renders the three sections with the selected decision highlighted" do
    list = render_inline(Organisms::InboxList::Component.new(inbox: Inbox::Sample.new("pr_8")))

    assert_equal 3, list.css("section h2").size
    assert_equal 4, list.css("section#decisions a").size
    assert_equal "true", list.css("a[aria-current]").attr("aria-current").value
    assert_includes list.css("a[aria-current]").text, "Create bare repositories on disk"
  end

  test "hides the decisions and working sections when empty" do
    inbox = Struct.new(:decisions, :working, :issues, :selected).new([], [], [], nil)
    list = render_inline(Organisms::InboxList::Component.new(inbox: inbox))

    assert_empty list.css("section#decisions")
    assert_empty list.css("section#working")
    assert_equal 1, list.css("section").size
    assert_includes list.text, "No tienes issues abiertos."
  end
end
