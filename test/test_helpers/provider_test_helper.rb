module ProviderTestHelper
  # Replaces the provider call for the block: pass an HTTP status code or an exception to raise.
  def with_provider_response(response)
    original = ProviderCredentials::Save.method(:list_models)
    ProviderCredentials::Save.define_singleton_method(:list_models) do |_provider, _api_key|
      response.is_a?(Exception) ? raise(response) : response
    end
    yield
  ensure
    ProviderCredentials::Save.define_singleton_method(:list_models, original)
  end
end
