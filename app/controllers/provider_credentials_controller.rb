class ProviderCredentialsController < ApplicationController
  rate_limit to: 10, within: 3.minutes, only: :create, with: -> { redirect_to new_provider_credential_path, alert: t("provider_credentials.create.rate_limited") }

  def new
    @provider_credential = Current.user.provider_credentials.new
  end

  def create
    @provider_credential = Current.user.provider_credentials.find_or_initialize_by(provider: :deepseek)
    @provider_credential.api_key = provider_credential_params[:api_key]

    if ProviderCredentials::Save.call(@provider_credential)
      redirect_to settings_path, notice: t(".#{@provider_credential.status}")
    else
      render :new, status: :unprocessable_entity
    end
  end

  def destroy
    Current.user.provider_credentials.find(params[:id]).destroy!
    redirect_to settings_path, notice: t(".destroyed"), status: :see_other
  end

  private
  def provider_credential_params
    params.expect(provider_credential: [:api_key])
  end
end
