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

  def update
    @access_token = Current.user.access_tokens.active.find(params[:id])
    saved = @access_token.update(scopes_params)

    respond_to do |format|
      format.turbo_stream { render status: (saved ? :ok : :unprocessable_entity) }
      format.html do
        if saved
          redirect_to access_tokens_path, notice: t(".updated"), status: :see_other
        else
          redirect_to access_tokens_path, alert: t("home.checklist.push.scopes_blank"), status: :see_other
        end
      end
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

  def scopes_params
    params.expect(access_token: [{ scopes: [] }])
  end
end
