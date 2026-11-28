require "test_helper"

class ProviderCredentials::SaveTest < ActiveSupport::TestCase
  def credential(api_key: "sk-test")
    users(:one).provider_credentials.new(api_key: api_key)
  end

  def save(credential, response)
    ProviderCredentials::Save.call(credential, request: ->(_provider, _api_key) { response.is_a?(Exception) ? raise(response) : response })
  end

  test "saves a key the provider accepts as valid" do
    record = credential

    assert save(record, 200)
    assert record.reload.status_valid?
    assert_not_nil record.validated_at
    assert_nil record.last_error
  end

  test "does not save a key the provider rejects" do
    record = credential

    assert_not save(record, 401)
    assert_not record.persisted?
    assert record.errors.added?(:api_key, :rejected)
  end

  test "saves the key as unverified on a timeout" do
    record = credential

    assert save(record, Net::OpenTimeout.new)
    assert record.reload.status_unverified?
    assert_equal "timeout", record.last_error
  end

  test "saves the key as unverified when the provider fails" do
    record = credential

    assert save(record, 503)
    assert record.reload.status_unverified?
    assert_equal "http_503", record.last_error
  end

  test "does not call the provider for an invalid record" do
    record = credential(api_key: "")

    assert_not ProviderCredentials::Save.call(record, request: ->(*) { flunk "provider called" })
  end

  test "calls the DeepSeek models endpoint with the key" do
    assert_equal URI("https://api.deepseek.com/models"), ProviderCredentials::Save::ENDPOINTS.fetch("deepseek")
  end
end
