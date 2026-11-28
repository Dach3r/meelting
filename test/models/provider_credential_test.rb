require "test_helper"

class ProviderCredentialTest < ActiveSupport::TestCase
  test "encrypts the api key and defaults to deepseek and unverified" do
    credential = users(:one).provider_credentials.create!(api_key: "  sk-secret  ")

    assert credential.deepseek?
    assert credential.status_unverified?
    assert_equal "sk-secret", credential.reload.api_key
    assert_not_includes ProviderCredential.connection.select_value("SELECT api_key FROM provider_credentials WHERE id = #{credential.id}"), "sk-secret"
  end

  test "requires an api key and one credential per provider" do
    assert_not ProviderCredential.new(user: users(:one), api_key: " ").valid?

    users(:one).provider_credentials.create!(api_key: "sk-one")
    duplicate = users(:one).provider_credentials.new(api_key: "sk-two")
    assert_not duplicate.valid?
    assert duplicate.errors[:provider].any?
  end

  test "only accepts deepseek as provider" do
    credential = ProviderCredential.new(user: users(:one), api_key: "sk", provider: "anthropic")

    assert_not credential.valid?
    assert credential.errors[:provider].any?
  end

  test "the database rejects unknown providers and statuses" do
    credential = users(:one).provider_credentials.create!(api_key: "sk")

    assert_raises(ActiveRecord::StatementInvalid) { credential.update_columns(provider: "anthropic") }
  end
end
