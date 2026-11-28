require "test_helper"

class Molecules::PushCommands::ComponentTest < ViewComponent::TestCase
  setup { @repository = repositories(:meelting) }

  test "shows the remote and push commands without credentials and the generate button" do
    node = render_inline(Molecules::PushCommands::Component.new(repository: @repository))

    commands = node.css("pre code").text
    assert_includes commands, "git remote add origin http://test.host/ada/meelting.git"
    assert_includes commands, "git push -u origin main"
    assert_equal "/repositories/meelting/push_token", node.css("form").first["action"]
    assert_empty node.css("#push_token")
  end

  test "shows the token apart from the commands, as the password for the username" do
    node = render_inline(Molecules::PushCommands::Component.new(repository: @repository, token: "mlt_secret"))

    assert_not_includes node.css("pre code").text, "mlt_secret"
    assert_equal %w[ada mlt_secret], node.css("#push_token dd").map(&:text)
    assert_includes node.css("#push_token").text, "credential helper"
    assert_empty node.css("form")
  end
end
