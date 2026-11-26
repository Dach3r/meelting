module Molecules
  class BrandComponent < ViewComponent::Base
    def call
      link_to t("navbar.brand"), root_path, class: "text-lg font-bold text-gray-900"
    end
  end
end
