class SettingsController < ApplicationController
  def show
    @provider_credential = Current.user.provider_credentials.first
  end
end
