module Molecules
  module PushCommands
    class Component < ViewComponent::Base
      REMOTE = "meelting".freeze

      def initialize(repository:, token: nil)
        @repository = repository
        @token = token
      end

      private
      attr_reader :repository, :token

      def remote_url
        base = URI(root_url)
        host = base.port == base.default_port ? base.host : "#{base.host}:#{base.port}"
        username = repository.user.username
        "#{base.scheme}://#{username}:#{token || t("home.checklist.push.token_placeholder")}@#{host}/#{username}/#{repository.slug}.git"
      end
    end
  end
end
