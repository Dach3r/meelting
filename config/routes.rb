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
  resources :access_tokens, only: %i[index new create destroy]
  get "up" => "rails/health#show", as: :rails_health_check

  constraints(AuthenticatedConstraint.new) do
    root "home#index"
  end

  root "auth/sessions#new", as: :unauthenticated_root
end
