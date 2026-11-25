class AccessTokensController < ApplicationController
  def index
    @access_tokens = Current.user.access_tokens.order(created_at: :desc)
    @plain_token = flash[:plain_token]
  end

  def new
    @access_token = Current.user.access_tokens.new
  end

  def create
    @access_token = Current.user.access_tokens.new(access_token_params)

    if @access_token.save
      flash[:plain_token] = @access_token.plain_token
      redirect_to access_tokens_path
    else
      render :new, status: :unprocessable_entity
    end
  end

  def destroy
    Current.user.access_tokens.find(params[:id]).revoke!
    redirect_to access_tokens_path, notice: t(".revoked"), status: :see_other
  end

  private
  def access_token_params
    params.expect(access_token: [:name, :expires_at, { scopes: [] }])
  end
end
