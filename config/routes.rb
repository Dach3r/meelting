class AuthenticatedConstraint
  def matches?(request)
    session_id = request.cookie_jar.signed[:session_id]
    session_id.present? && Session.exists?(id: session_id)
  end
end

Rails.application.routes.draw do
  scope module: :auth do
    resource :session
    resource :registration, only: %i[new create]
    resources :passwords, param: :token
  end

  constraints(AuthenticatedConstraint.new) do
    root "home#index"
  end

  root "landing#show", as: :unauthenticated_root

  get "up" => "rails/health#show", as: :rails_health_check

  resources :access_tokens, only: %i[index new create destroy]
  resources :repositories, only: %i[index new create]
end
