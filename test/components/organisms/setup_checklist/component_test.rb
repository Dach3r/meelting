require "test_helper"

class Organisms::SetupChecklist::ComponentTest < ViewComponent::TestCase
  def steps(done_keys)
    %i[access_token repository mirror model_key overview first_issue].map { |key_| Workspace::ChecklistStep.new(key: key_, done: done_keys.include?(key_)) }
  end

  test "renders six steps and progress" do
    node = render_inline(Organisms::SetupChecklist::Component.new(steps: steps([:access_token])))

    assert_equal 6, node.css("ol li").size
    assert_includes node.text, "1 de 6"
  end

  test "fills the progress ring by completed steps" do
    node = render_inline(Organisms::SetupChecklist::Component.new(steps: steps(%i[access_token repository mirror])))

    assert_equal "471.24 942.48", node.css("svg circle").last["stroke-dasharray"]
    assert_includes node.text, "de 6"
  end

  test "highlights the first pending step with its action" do
    node = render_inline(Organisms::SetupChecklist::Component.new(steps: steps([:access_token])))

    assert_equal 1, node.css("li[aria-current=step]").size
    assert_equal "/repositories/new", node.css("li[aria-current=step] a").first["href"]
  end

  test "marks done steps accessibly" do
    node = render_inline(Organisms::SetupChecklist::Component.new(steps: steps([:access_token])))

    assert_equal 1, node.css("[role=img][aria-label=Hecho]").size
  end
end
