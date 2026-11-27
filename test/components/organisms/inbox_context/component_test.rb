require "test_helper"

class Organisms::InboxContext::ComponentTest < ViewComponent::TestCase
  test "shows push mirror state per repository" do
    context = render_inline(Organisms::InboxContext::Component.new(inbox: Inbox::Sample.new))

    assert_includes context.text, "david/meelting"
    assert_includes context.text, "Sincronizado"
    assert_includes context.text, "Push fallido"
  end

  test "marks memory proposed by an agent" do
    context = render_inline(Organisms::InboxContext::Component.new(inbox: Inbox::Sample.new))

    assert_includes context.text, "decisions/0004-bare-repos-on-disk.md"
    assert_includes context.text, "propuesta"
  end
end
