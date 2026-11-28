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

  test "opens the token in a modal apart from the commands, as the password for the username" do
    token = users(:one).access_tokens.create!(name: "push", scopes: %w[git_read git_write])
    node = render_inline(Molecules::PushCommands::Component.new(repository: @repository, access_token: token))

    assert_not_includes node.css("pre code").text, token.plain_token
    assert_equal ["ada", token.plain_token], node.css("#push_token code").map(&:text)
    assert_includes node.css("#push_token").text, "credential helper"
    assert_equal "dialog", node.css("dialog#push_token").first["data-controller"]
    assert_equal 2, node.css("form").size
    assert_equal %w[git_read git_write], node.css("#push_token input[type=checkbox][checked]").map { _1["value"] }
    assert_equal 3, node.css("#push_token input[type=checkbox]").size
    assert_equal "/access_tokens/#{token.id}", node.css("#push_token form").first["action"]
    assert_equal "Aceptar", node.css("#push_token form button[type=submit]").text
  end
end
