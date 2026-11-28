require "test_helper"

class Molecules::TokenScopes::ComponentTest < ViewComponent::TestCase
  test "checks the token's scopes and links to the token list" do
    node = render_inline(Molecules::TokenScopes::Component.new(access_token: AccessToken.new(scopes: %w[git_write])))

    assert_equal %w[git_write], node.css("input[type=checkbox][checked]").map { _1["value"] }
    assert_equal "/access_tokens", node.css("a").first["href"]
    assert_empty node.css("[role=alert]")
  end

  test "asks for at least one scope when none is chosen" do
    token = AccessToken.new(scopes: [])
    token.validate

    node = render_inline(Molecules::TokenScopes::Component.new(access_token: token))

    assert_equal "Elige al menos un permiso.", node.css("[role=alert]").text
  end
end
