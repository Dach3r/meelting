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
    resource :username_availability, only: :show

    resources :passwords, param: :token
  end

  constraints(AuthenticatedConstraint.new) do
    root "home#show"
  end

  root "landing#show", as: :unauthenticated_root

  get "up" => "rails/health#show", as: :rails_health_check

  resources :access_tokens, only: %i[index new create update destroy]
  resources :repositories, only: %i[index new create]
  scope ":username", constraints: { username: /[a-z][a-z0-9_]*/ } do
    resources :repositories, only: [], param: :slug do
      get "blob/*path", to: "repositories/blobs#show", as: :blob, format: false
      get ":section", to: "repositories/sections#show", as: :section, constraints: { section: /issues|pull_requests|runs|memory/ }
      resource :push_token, only: :create, module: :repositories
    end
    get "repositories/:repository_slug", to: "repositories/sections#show", as: :repository, defaults: { section: "code" }
  end

  resources :provider_credentials, only: %i[new create destroy]
  resources :runs, only: :index
  resource :settings, only: :show
  resource :sidebar_expansion, only: :update, controller: "navigation/expansions"
end
