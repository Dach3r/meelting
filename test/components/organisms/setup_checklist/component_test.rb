require "test_helper"

class Organisms::SetupChecklist::ComponentTest < ViewComponent::TestCase
  def steps(done_keys)
    %i[repository model_key overview].map { |key_| Workspace::ChecklistStep.new(key: key_, done: done_keys.include?(key_)) } +
      [Workspace::ChecklistStep.new(key: :first_issue, done: false, available: false)]
  end

  def render_checklist(done_keys = [], repository: nil)
    render_inline(Organisms::SetupChecklist::Component.new(steps: steps(done_keys), repository: repository))
  end

  test "renders four steps and progress" do
    node = render_checklist([:repository])

    assert_equal 4, node.css("ol li").size
    assert_includes node.text, "1 de 4"
  end

  test "highlights the current station and shows its detail below the route" do
    node = render_checklist(%i[repository model_key])

    assert_includes node.css("ol li[aria-current=step]").text, "Escribe .project/overview.md"
    assert_includes node.css("#current_step h2").text, "Escribe .project/overview.md"
  end

  test "hides the detail panel when no step is pending" do
    node = render_checklist(%i[repository model_key overview])

    assert_empty node.css("#current_step")
  end

  test "asks to create a repository when there is none" do
    node = render_checklist

    assert_equal 1, node.css("li[aria-current=step]").size
    assert_equal "/repositories/new", node.css("#current_step a").first["href"]
    assert_empty node.css("#push_commands")
  end

  test "shows the push commands with the token button once a repository exists" do
    node = render_checklist(repository: repositories(:meelting))

    assert_includes node.css("#current_step #push_commands code").text, "git push -u origin main"
    assert_equal "/repositories/meelting/push_token", node.css("#current_step form").first["action"]
  end

  test "links the model key step to the key form when it is current" do
    node = render_checklist([:repository])

    assert_equal "/provider_credentials/new", node.css("#current_step a").first["href"]
    assert_includes node.css("#current_step").text, "DeepSeek"
  end

  test "offers the overview template when it is the current step" do
    node = render_checklist(%i[repository model_key])

    template = node.css("details pre").text
    assert_includes template, "## Qué hace el sistema"
    assert_includes template, "## Para quién"
    assert_includes template, "## Módulos principales"
    assert_empty render_checklist.css("details")
  end

  test "disables the first issue step until Phases 2 and 3" do
    node = render_checklist(%i[repository model_key overview])

    disabled = node.css("li[aria-disabled=true]")
    assert_equal 1, disabled.size
    assert_includes disabled.text, "Disponible en Fase 2/3"
    assert_empty node.css("li[aria-current=step]")
  end

  test "marks done steps accessibly" do
    node = render_checklist([:repository])

    assert_equal 1, node.css("[role=img][aria-label=Hecho]").size
  end
end
