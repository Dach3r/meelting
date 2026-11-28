require "test_helper"

class Molecules::PushCommands::ComponentTest < ViewComponent::TestCase
  setup { @repository = repositories(:meelting) }

  test "shows the remote and push commands with a token placeholder and the generate button" do
    node = render_inline(Molecules::PushCommands::Component.new(repository: @repository))

    assert_includes node.css("code").text, "git remote add meelting http://ada:TOKEN@test.host/ada/meelting.git"
    assert_includes node.css("code").text, "git push -u meelting main"
    assert_equal "/repositories/meelting/push_token", node.css("form").first["action"]
    assert_includes node.text, "Generar token"
  end

  test "inserts the token in the commands and drops the button" do
    node = render_inline(Molecules::PushCommands::Component.new(repository: @repository, token: "mlt_secret"))

    assert_includes node.css("code").text, "http://ada:mlt_secret@test.host/ada/meelting.git"
    assert_empty node.css("form")
    assert_includes node.css("[role=status]").text, "no se volverá a mostrar"
  end
end
