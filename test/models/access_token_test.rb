require "test_helper"

class AccessTokenTest < ActiveSupport::TestCase
  setup { @user = users(:one) }

  test "generates a prefixed token and stores only its digest" do
    token = @user.access_tokens.create!(name: "CI", scopes: %w[api])

    assert token.plain_token.start_with?(AccessToken::PREFIX)
    assert_equal AccessToken.digest(token.plain_token), token.token_digest
    assert_not_equal token.plain_token, token.reload.token_digest
    assert_nil AccessToken.find(token.id).plain_token
  end

  test "requires name" do
    token = @user.access_tokens.new(scopes: %w[api])
    assert_not token.valid?
    assert token.errors[:name].any?
  end

  test "requires at least one scope" do
    token = @user.access_tokens.new(name: "CI", scopes: [])
    assert_not token.valid?
    assert token.errors[:scopes].any?
  end

  test "rejects unknown scopes" do
    token = @user.access_tokens.new(name: "CI", scopes: %w[api admin])
    assert_not token.valid?
    assert token.errors[:scopes].any?
  end

  test "ignores blank and duplicate scopes" do
    token = @user.access_tokens.new(name: "CI", scopes: ["", "api", "api"])
    assert token.valid?
    assert_equal %w[api], token.scopes
  end

  test "active excludes revoked and expired tokens" do
    assert_includes AccessToken.active, access_tokens(:git)
    assert_not_includes AccessToken.active, access_tokens(:revoked)
    assert_not_includes AccessToken.active, access_tokens(:expired)
  end

  test "authenticate returns the token for a valid plain token and scope" do
    assert_equal access_tokens(:git), AccessToken.authenticate("mlt_fixture_git", scope: :git_write)
  end

  test "authenticate rejects a scope the token lacks" do
    assert_nil AccessToken.authenticate("mlt_fixture_git", scope: :api)
  end

  test "authenticate rejects unknown, blank, revoked and expired tokens" do
    assert_nil AccessToken.authenticate("mlt_nope", scope: :git_read)
    assert_nil AccessToken.authenticate("", scope: :git_read)
    assert_nil AccessToken.authenticate(nil, scope: :git_read)
    assert_nil AccessToken.authenticate("mlt_fixture_revoked", scope: :git_read)
    assert_nil AccessToken.authenticate("mlt_fixture_expired", scope: :git_read)
  end

  test "authenticate records last use, throttled" do
    token = access_tokens(:git)
    assert_nil token.last_used_at

    AccessToken.authenticate("mlt_fixture_git", scope: :git_read)
    first_use = token.reload.last_used_at
    assert first_use

    AccessToken.authenticate("mlt_fixture_git", scope: :git_read)
    assert_equal first_use, token.reload.last_used_at
  end

  test "revoke! sets revoked_at once" do
    token = access_tokens(:git)
    token.revoke!
    revoked_at = token.reload.revoked_at
    assert revoked_at

    token.revoke!
    assert_equal revoked_at, token.reload.revoked_at
  end

  test "destroying user destroys access tokens" do
    assert_difference "AccessToken.count", -3 do
      @user.destroy
    end
  end
end
