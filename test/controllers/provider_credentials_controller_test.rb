require "test_helper"

class ProviderCredentialsControllerTest < ActionDispatch::IntegrationTest
  setup { sign_in_as users(:one) }

  test "requires authentication" do
    sign_out
    get new_provider_credential_path

    assert_redirected_to new_session_path
  end

  test "saves a verified key" do
    with_provider_response(200) do
      post provider_credentials_path, params: { provider_credential: { api_key: "sk-good" } }
    end

    assert_redirected_to settings_path
    assert users(:one).provider_credentials.sole.status_valid?
    follow_redirect!
    assert_select "#model_keys", /DeepSeek: verificada/
  end

  test "replaces the existing key" do
    users(:one).provider_credentials.create!(api_key: "sk-old")

    with_provider_response(200) do
      post provider_credentials_path, params: { provider_credential: { api_key: "sk-new" } }
    end

    assert_equal "sk-new", users(:one).provider_credentials.sole.api_key
  end

  test "rejects a key the provider refuses" do
    with_provider_response(401) do
      post provider_credentials_path, params: { provider_credential: { api_key: "sk-bad" } }
    end

    assert_response :unprocessable_entity
    assert_select "#error_explanation", /rechazada por el proveedor/
    assert_empty users(:one).provider_credentials
  end

  test "keeps the key unverified when the provider times out" do
    with_provider_response(Net::ReadTimeout.new) do
      post provider_credentials_path, params: { provider_credential: { api_key: "sk-slow" } }
    end

    assert users(:one).provider_credentials.sole.status_unverified?
    assert_equal I18n.t("provider_credentials.create.unverified"), flash[:notice]
    follow_redirect!
    assert_select "#model_keys [role=status]", /No pudimos verificarla/
    assert_select "#model_keys a", "Reintentar"
  end

  test "removes only the current user's key" do
    credential = users(:one).provider_credentials.create!(api_key: "sk")
    other = users(:two).provider_credentials.create!(api_key: "sk")

    delete provider_credential_path(other)
    assert_response :not_found

    delete provider_credential_path(credential)
    assert_redirected_to settings_path
    assert_not ProviderCredential.exists?(credential.id)
  end
end
