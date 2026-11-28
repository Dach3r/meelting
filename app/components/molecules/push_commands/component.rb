module Molecules
  module PushCommands
    class Component < ViewComponent::Base
      def initialize(repository:, access_token: nil)
        @repository = repository
        @access_token = access_token
      end

      private
      attr_reader :repository, :access_token

      def username
        repository.user.username
      end

      def remote_url
        URI.join(root_url, "#{username}/#{repository.slug}.git").to_s
      end
    end
  end
end
