module Repositories
  class PushTokensController < ApplicationController
    SCOPES = %w[git_read git_write].freeze

    def create
      @repository = Current.user.repositories.active.find_by!(slug: params[:repository_slug])
      @access_token = Current.user.access_tokens.create!(name: t(".name", repository: @repository.name), scopes: SCOPES)

      respond_to do |format|
        format.turbo_stream
        format.html do
          flash[:plain_token] = @access_token.plain_token
          redirect_to access_tokens_path
        end
      end
    end
  end
end
