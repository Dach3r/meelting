module Molecules
  module PushCommands
    class Component < ViewComponent::Base
      def initialize(repository:, token: nil)
        @repository = repository
        @token = token
      end

      private
      attr_reader :repository, :token

      def username
        repository.user.username
      end

      def remote_url
        URI.join(root_url, "#{username}/#{repository.slug}.git").to_s
      end
    end
  end
end
