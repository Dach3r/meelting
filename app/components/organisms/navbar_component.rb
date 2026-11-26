module Organisms
  class NavbarComponent < ViewComponent::Base
    def initialize(authenticated:)
      @authenticated = authenticated
    end

    private
    attr_reader :authenticated
  end
end
